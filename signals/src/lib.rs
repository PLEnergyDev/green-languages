pub mod signal;

#[unsafe(no_mangle)]
pub extern "C" fn start_gl() -> i32 {
    signal::start_gl()
}

#[unsafe(no_mangle)]
pub extern "C" fn end_gl() {
    signal::end_gl()
}

#[cfg(target_os = "linux")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod jni {
    use jni::objects::JClass;
    use jni::sys::jint;
    use jni::JNIEnv;

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_Signals_startGl(
        _env: JNIEnv,
        _class: JClass,
    ) -> jint {
        crate::signal::start_gl()
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_Signals_endGl(_env: JNIEnv, _class: JClass) {
        crate::signal::end_gl();
    }
}
