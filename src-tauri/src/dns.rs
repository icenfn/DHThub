//! 自定义 DNS 解析：用 hickory-resolver 覆盖 reqwest 的域名解析（保留 TLS SNI）

use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
struct CustomDnsResolver(TokioAsyncResolver);

impl Resolve for CustomDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.0.clone();
        Box::pin(async move {
            let lookup = resolver
                .lookup_ip(name.as_str())
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            let addrs: Addrs = Box::new(lookup.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addrs)
        })
    }
}

/// 测速用解析选项：快速失败（2s 超时、1 次尝试），UDP 失败自动走 TCP 兜底
fn speed_opt() -> ResolverOpts {
    ResolverOpts {
        timeout: Duration::from_secs(2),
        attempts: 1,
        ..Default::default()
    }
}

/// 构造 UDP + TCP 双通道的解析配置（部分网络屏蔽 UDP:53，TCP 兜底）
fn dns_config(addr: SocketAddr) -> ResolverConfig {
    let mut cfg = ResolverConfig::new();
    cfg.add_name_server(NameServerConfig::new(addr, Protocol::Udp));
    cfg.add_name_server(NameServerConfig::new(addr, Protocol::Tcp));
    cfg
}

/// 用指定 DNS 服务器解析 example.com 并返回耗时（毫秒）；失败返回 Err
pub async fn measure_latency(dns: &str) -> Result<u64, String> {
    let addr: SocketAddr = format!("{dns}:53")
        .parse()
        .map_err(|_| format!("DNS 地址无效：{dns}"))?;
    let resolver = TokioAsyncResolver::tokio(dns_config(addr), speed_opt());
    let started = std::time::Instant::now();
    let _ = resolver
        .lookup_ip("example.com")
        .await
        .map_err(|e| e.to_string())?;
    Ok(started.elapsed().as_millis() as u64)
}

/// 构建使用指定 DNS 服务器的 HTTP 客户端（与默认客户端相同 UA / 超时）
pub fn build_client(dns: &str) -> Result<reqwest::Client, String> {
    let addr: SocketAddr = format!("{dns}:53")
        .parse()
        .map_err(|_| format!("DNS 地址无效：{dns}"))?;
    // 搜索用解析：UDP + TCP 兜底，3s 超时 1 次尝试（避免 DNS 异常拖垮整批搜索）
    let resolver = TokioAsyncResolver::tokio(
        dns_config(addr),
        ResolverOpts {
            timeout: Duration::from_secs(3),
            attempts: 1,
            ..Default::default()
        },
    );
    // reqwest 0.12 的 dns_resolver 需要具体类型（非 dyn），直接传入具体 resolver
    let resolver = Arc::new(CustomDnsResolver(resolver));
    reqwest::Client::builder()
        .user_agent(format!(
            "DHThub/{} (+https://github.com/icenfn/DHThub)",
            env!("CARGO_PKG_VERSION")
        ))
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .dns_resolver(resolver)
        .build()
        .map_err(|e| e.to_string())
}
