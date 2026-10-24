/**
 * CSS 级联顺序兼容层（Vuetify 4 必需；对 Chromium 91 (Android WebView 91) 更是硬性要求）。
 *
 * ## 背景
 * Vuetify 4 的样式基于 CSS 级联层（`@layer`）组织：
 *   @layer vuetify-core, vuetify-components, vuetify-pages, vuetify-overrides, vuetify-utilities;
 * 主题工具类（`.bg-primary` / `.text-primary` …）在 **运行时** 由 JS 生成，整段包在
 * `@layer vuetify-utilities { … }` 里写进 `<style id="vuetify-theme-stylesheet">`。
 *
 * ## 问题
 * 构建产物要兼容 Android WebView 91（Chromium 91），它不认识 `@layer`：
 *  - 静态 CSS 已由 PostCSS（见 postcss.config.js）在构建期展平；
 *  - 运行时主题表不过 PostCSS，仍然带层。
 * 于是两边处于不同的「层世界」：CSS 规范规定**未分层规则优先于一切分层规则**，
 * 主题色背景永远被组件自带的 surface 背景盖掉（v-chip / v-switch / v-btn 不跟主题色）。
 * 在老引擎上更糟：`@layer` 整段被丢弃，`--v-theme-*` 变量全部消失。
 *
 * ## 处理（对**所有**引擎执行，保证新旧行为一致）
 *  1. 把运行时主题表里的 `@layer` 展平，两边都回到「无层级」世界，
 *     恢复传统的「特异性 + 源码顺序」比较；
 *  2. 展平后把主题表移到 `<head>` 末尾 —— Vuetify 原本把 utilities 排在层序最后，
 *     等于「同特异性时最后生效」，移到末尾正是复刻该语义。
 */

/** 与 Vuetify 运行时主题样式表一致的 DOM id。 */
const STYLESHEET_ID = 'vuetify-theme-stylesheet'

/**
 * 移除一段 CSS 中所有 `@layer` 构造：块形式把内容原位内联，语句形式
 * （`@layer a, b;`）直接丢弃。花括号感知，因此嵌套 layer 与嵌套规则都完好。
 */
export function stripCascadeLayers(css: string): string {
  let out = ''
  let i = 0
  const n = css.length
  while (i < n) {
    const at = css.indexOf('@layer', i)
    if (at === -1) {
      out += css.slice(i)
      break
    }
    out += css.slice(i, at)
    // 跳过层名列表，直到 `{` 或 `;`。
    let k = at + '@layer'.length
    while (k < n && css[k] !== '{' && css[k] !== ';') k++
    if (k >= n) {
      // 结尾残缺的 `@layer`：原样保留并结束。
      out += css.slice(at)
      break
    }
    if (css[k] === ';') {
      // `@layer a, b;` 声明语句：层已不存在，顺序无意义，直接丢弃。
      i = k + 1
      continue
    }
    // 块形式：找到配对的右花括号，把内部递归去层后内联。
    let depth = 0
    let m = k
    for (; m < n; m++) {
      const c = css[m]
      if (c === '{') depth++
      else if (c === '}') {
        depth--
        if (depth === 0) break
      }
    }
    const inner = css.slice(k + 1, m < n ? m : n)
    out += stripCascadeLayers(inner)
    i = m < n ? m + 1 : n
  }
  return out
}

/** 若某个 `<style>` 仍含 `@layer`，就地去层。 */
function flattenStyleElement(el: HTMLStyleElement): boolean {
  const css = el.textContent
  if (!css || css.indexOf('@layer') === -1) return false
  const flattened = stripCascadeLayers(css)
  if (!flattened || flattened === css) return false
  el.textContent = flattened
  return true
}

/**
 * 把主题样式表移到 `<head>` 末尾。
 * 去层之后所有规则同处「无层级」世界，同特异性时后出现者胜；utilities 本来排在
 * 层序最后，所以把主题表放到最后才能让 `.bg-primary` 压过组件的 surface 背景。
 */
function hoistThemeStylesheet(): void {
  if (typeof document === 'undefined') return
  const head = document.head
  const el = document.getElementById(STYLESHEET_ID)
  if (!head || !el) return
  if (head.lastElementChild !== el) head.appendChild(el)
}

/** 安装级联顺序兼容层。幂等；MutationObserver 会跟随 Vuetify 每次主题切换重建样式表。 */
export function installCascadeOrderCompat(): void {
  if (typeof document === 'undefined') return

  const processNode = (el: HTMLStyleElement): void => {
    // 去层后必须重新置后，否则后注入的静态 CSS 又可能排到它后面。
    if (flattenStyleElement(el)) hoistThemeStylesheet()
  }

  const processTree = (root: ParentNode): void => {
    root.querySelectorAll('style').forEach((node) => processNode(node as HTMLStyleElement))
    const known = document.getElementById(STYLESHEET_ID)
    if (known) processNode(known as HTMLStyleElement)
  }

  const start = (): void => {
    processTree(document)
    hoistThemeStylesheet()

    const head = document.head
    if (!head || typeof MutationObserver === 'undefined') return

    const observer = new MutationObserver((mutations) => {
      let themeTouched = false
      for (const mutation of mutations) {
        if (mutation.type === 'childList') {
          mutation.addedNodes.forEach((node) => {
            if (node.nodeType !== 1) return
            const el = node as Element
            if (el.id === STYLESHEET_ID || el.tagName === 'STYLE') {
              if (flattenStyleElement(el as HTMLStyleElement)) themeTouched = true
              if (el.id === STYLESHEET_ID) themeTouched = true
            } else {
              processTree(el)
            }
          })
        } else if (mutation.type === 'characterData') {
          const parent = mutation.target.parentNode as Element | null
          if (parent && parent.tagName === 'STYLE') {
            if (flattenStyleElement(parent as HTMLStyleElement)) themeTouched = true
            if (parent.id === STYLESHEET_ID) themeTouched = true
          }
        }
      }
      if (themeTouched) hoistThemeStylesheet()
    })

    // 监听 <head>：Vuetify 每次主题变化都会重建/覆写这个 <style>。
    observer.observe(head, { childList: true, subtree: true, characterData: true })
  }

  if (document.head) start()
  else document.addEventListener('DOMContentLoaded', start, { once: true })
}
