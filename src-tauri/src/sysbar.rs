//! Android 系统状态栏 / 导航栏颜色与图标明暗：跟随应用主题（桌面端不编译本模块）
//! 通过 JNI 调用 Activity 的 Window API；仅在 Android 构建中编译

use jni::objects::{JObject, JValue};
use jni::sys::{jint, jobject};
use jni::JavaVM;

/// 主题 surface 色：深色 #121212 / 浅色 #FFFFFF（与 Vuetify MD3 默认一致）
const COLOR_DARK: jint = 0xFF121212_u32 as jint;
const COLOR_LIGHT: jint = -1; // 0xFFFFFFFF
/// SYSTEM_UI_FLAG_LIGHT_STATUS_BAR（0x2000）| SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR（0x10）
const FLAG_LIGHT: jint = 0x2000 | 0x10;

fn with_env<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut jni::JNIEnv) -> Result<(), jni::errors::Error>,
{
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    f(&mut env).map_err(|e| e.to_string())
}

pub fn apply(dark: bool) -> Result<(), String> {
    with_env(|env| {
        let ctx = ndk_context::android_context();
        let activity = unsafe { JObject::from_raw(ctx.context().cast() as jobject) };

        let window = env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?
            .l()?;

        // 状态栏 / 导航栏背景色 = 主题 surface 色
        let color: jint = if dark { COLOR_DARK } else { COLOR_LIGHT };
        env.call_method(&window, "setStatusBarColor", "(I)V", &[JValue::Int(color)])?;
        env.call_method(
            &window,
            "setNavigationBarColor",
            "(I)V",
            &[JValue::Int(color)],
        )?;

        // 图标明暗：浅色主题 → 深色图标
        let decor = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?
            .l()?;
        let flags: jint = if dark { 0 } else { FLAG_LIGHT };
        env.call_method(
            &decor,
            "setSystemUiVisibility",
            "(I)V",
            &[JValue::Int(flags)],
        )?;

        Ok(())
    })
}
