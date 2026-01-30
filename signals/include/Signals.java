public class Signals {
    static {
        System.loadLibrary("signals");
    }
    public native int startGl();
    public native void endGl();
}
