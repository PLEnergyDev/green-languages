public class Iterations {
    static {
        System.loadLibrary("iterations");
    }
    public native int nextIteration();
    public native void markEnd();
}
