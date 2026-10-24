package com.dhthub.app

import android.content.res.Configuration
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

/**
 * DHThub 原生入口。
 *
 * 沉浸式状态栏 + 输入法（IME）避让方案参考 icenfn/astrbot-plus：
 *
 *  1. 用 WindowCompat.setDecorFitsSystemWindows(window, false) 全程 edge-to-edge，
 *     并且 **不给 decorView 加 padding** —— 一旦系统本身已按刘海 / 手势条内缩，
 *     再补一次 padding 就会出现「双层黑边 / 顶部留白过大」。
 *  2. 系统栏（含刘海）与软键盘的 inset 只通过一次 JS 桥接下发到 WebView，
 *     写成 CSS 变量 --safe-area-inset-* 与 --keyboard-height，由样式层统一消费。
 *  3. 键盘打开时，底部导航条高度必须扣除，避免「手势条 + 键盘」各留一份空隙。
 *  4. 状态栏图标颜色跟随主题（浅色主题用深色图标），否则纯白背景上图标会看不清。
 */
class MainActivity : TauriActivity() {

  // 最近一次收到的系统栏 / 刘海 insets（CSS px），用于 WebView 重建或重新获得焦点时补发
  private var insetTop = 0
  private var insetBottom = 0
  private var insetLeft = 0
  private var insetRight = 0

  // 软键盘高度（CSS px），隐藏时为 0
  private var keyboardHeight = 0

  // JS 桥接是否已挂到 WebView 上
  private var bridgeAdded = false

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    // ---- edge-to-edge，inset 只下发一次 ----
    WindowCompat.setDecorFitsSystemWindows(window, false)
    makeSystemBarsTransparent()

    val decor = window.decorView
    ViewCompat.setOnApplyWindowInsetsListener(decor) { _, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      // 软键盘作为独立的 inset 到达。应用是 edge-to-edge 且 targetSdk 较高时，
      // WebView 不会因键盘而改变布局高度，因此由这个值驱动前端把输入区抬起来。
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      applyInsets(bars.top, bars.bottom, bars.left, bars.right, ime)
      insets
    }
    ViewCompat.requestApplyInsets(decor)

    // 首次 inset 到达时 WebView 可能还没创建，短暂重试补发
    for (delay in longArrayOf(300L, 900L, 1800L)) {
      decor.postDelayed({ deliverInsets() }, delay)
    }
  }

  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) {
      // 从输入法返回等焦点变化可能改变 inset
      ViewCompat.requestApplyInsets(window.decorView)
      deliverInsets()
    }
  }

  /** 系统栏透明，让 WebView 内容真正绘制到状态栏 / 导航栏下方 */
  private fun makeSystemBarsTransparent() {
    window.statusBarColor = Color.TRANSPARENT
    window.navigationBarColor = Color.TRANSPARENT
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      window.isStatusBarContrastEnforced = false
      window.isNavigationBarContrastEnforced = false
    }
    // 先按系统夜间模式给一个初始值，前端拿到用户主题后会通过桥接再校正
    val nightMask = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
    applyBarAppearance(nightMask != Configuration.UI_MODE_NIGHT_YES)
  }

  /**
   * 切换状态栏 / 导航栏图标颜色。
   *
   * light = true → 深色图标（用于浅色背景）；false → 浅色图标（用于深色背景）。
   * edge-to-edge + 透明栏下，平台默认是浅色图标，放在纯白背景上几乎不可见
   * （即"状态栏不显示"的成因），因此必须跟随主题切换。
   */
  private fun applyBarAppearance(light: Boolean) {
    runOnUiThread {
      val controller = WindowInsetsControllerCompat(window, window.decorView)
      controller.isAppearanceLightStatusBars = light
      controller.isAppearanceLightNavigationBars = light
    }
  }

  /** 暴露给 WebView 的桥接对象：window.DHThubNative */
  private inner class NativeBridge {
    /** 前端确定主题后调用，修正状态栏图标明暗 */
    @android.webkit.JavascriptInterface
    fun setStatusBarAppearance(light: Boolean) {
      applyBarAppearance(light)
    }

    /** 把任务切到后台但不结束 Activity（返回桌面仍驻留） */
    @android.webkit.JavascriptInterface
    fun moveToBackground() {
      runOnUiThread { moveTaskToBack(true) }
    }
  }

  /** 保存 inset（物理 px 转 CSS px）并推给 WebView */
  private fun applyInsets(topPx: Int, bottomPx: Int, leftPx: Int, rightPx: Int, imePx: Int) {
    val density = resources.displayMetrics.density.takeIf { it > 0f } ?: 1f
    insetTop = (topPx / density).toInt()
    insetLeft = (leftPx / density).toInt()
    insetRight = (rightPx / density).toInt()

    val barsBottomCss = (bottomPx / density).toInt()
    val imeCss = (imePx / density).toInt()

    // 软键盘会盖住导航栏，两者不能同时扣减：
    // 只暴露键盘没有覆盖的那部分，键盘打开时为 0（不产生额外空隙）。
    insetBottom = (barsBottomCss - imeCss).coerceAtLeast(0)
    keyboardHeight = imeCss

    deliverInsets()
  }

  private fun deliverInsets() {
    val web = findWebView() ?: return
    if (!bridgeAdded) {
      web.addJavascriptInterface(NativeBridge(), "DHThubNative")
      bridgeAdded = true
    }
    val js =
      "window.__dhthubApplyInsets&&window.__dhthubApplyInsets($insetTop,$insetBottom,$insetLeft,$insetRight,$keyboardHeight);"
    web.post { web.evaluateJavascript(js, null) }
  }

  private fun findWebView(): android.webkit.WebView? {
    var web: android.webkit.WebView? = null
    fun scan(v: View) {
      if (web != null) return
      if (v is android.webkit.WebView) {
        web = v
        return
      }
      if (v is android.view.ViewGroup) {
        for (i in 0 until v.childCount) scan(v.getChildAt(i))
      }
    }
    scan(window.decorView)
    return web
  }

  // 硬件返回键：派发 `android:back` 事件让前端决定返回 / 关闭，而不是直接结束 Activity
  @Deprecated("Deprecated in Java")
  override fun onBackPressed() {
    notifyWebviewBack()
  }

  private fun notifyWebviewBack() {
    try {
      val web = findWebView() ?: return finish()
      web.post {
        web.evaluateJavascript(
          "window.dispatchEvent(new CustomEvent('android:back'))",
          null,
        )
      }
    } catch (_: Exception) {
      finish()
    }
  }
}
