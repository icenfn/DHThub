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

/// 用指定 DNS 服务器解析 example.com 并返回耗时（毫秒）；失败返回 Err
pub async fn measure_latency(dns: &str) -> Result<u64, String> {
    let addr: SocketAddr = format!("{dns}:53")
        .parse()
        .map_err(|_| format!("DNS 地址无效：{dns}"))?;
    let mut cfg = ResolverConfig::new();
    cfg.add_name_server(NameServerConfig::new(addr, Protocol::Udp));
    let resolver = TokioAsyncResolver::tokio(cfg, ResolverOpts::default());
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
    let mut cfg = ResolverConfig::new();
    cfg.add_name_server(NameServerConfig::new(addr, Protocol::Udp));
    let resolver = TokioAsyncResolver::tokio(cfg, ResolverOpts::default());
    let resolver: Arc<dyn Resolve> = Arc::new(CustomDnsResolver(resolver));
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
