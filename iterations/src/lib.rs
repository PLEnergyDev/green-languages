pub mod share;
pub mod signal;

#[no_mangle]
pub extern "C" fn next_iteration() -> i32 {
    signal::next_iteration()
}

#[no_mangle]
pub extern "C" fn mark_end() {
    signal::mark_end()
}

// JNI interface for Java
#[cfg(target_os = "linux")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod jni {
    use jni::objects::JClass;
    use jni::sys::jint;
    use jni::JNIEnv;

    #[no_mangle]
    pub extern "system" fn Java_Iterations_nextIteration(_env: JNIEnv, _class: JClass) -> jint {
        crate::signal::next_iteration()
    }

    #[no_mangle]
    pub extern "system" fn Java_Iterations_markEnd(_env: JNIEnv, _class: JClass) {
        crate::signal::mark_end();
    }
}
