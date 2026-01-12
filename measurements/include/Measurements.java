public class Measurements {
    static {
        System.loadLibrary("measurements");
    }
    public native int startMeasurement();
    public native void endMeasurement();
}
