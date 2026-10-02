// 调试模式工具：由 settings 存储层同步开关状态，避免循环依赖

let enabled = false

/** 由设置存储层在加载/变更时同步调试开关 */
export function setDebug(v: boolean) {
  enabled = v
}

/** 调试模式下输出带前缀的日志；关闭时静默 */
export function debugLog(...args: unknown[]): void {
  if (enabled) console.info('[DHThub]', ...args)
}
