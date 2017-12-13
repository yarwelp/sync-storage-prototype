package com.mozilla.toodle.rust;

import com.sun.jna.Pointer;
import com.sun.jna.Structure;

import java.io.Closeable;
import java.io.IOException;
import java.util.Arrays;
import java.util.List;

public class NativeResult extends Structure implements Closeable {
    public static class ByReference extends NativeItem implements Structure.ByReference {
    }

    public static class ByValue extends NativeItem implements Structure.ByValue {
    }

    public Pointer obj;
    public String error;

    @Override
    protected List<String> getFieldOrder() {
        return Arrays.asList("obj", "error");
    }

    @Override
    public void close() throws IOException {

    }
}
