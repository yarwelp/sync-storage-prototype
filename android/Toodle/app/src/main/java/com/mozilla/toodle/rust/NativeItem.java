package com.mozilla.toodle.rust;

import android.util.Log;

import com.sun.jna.IntegerType;
import com.sun.jna.NativeLong;
import com.sun.jna.Structure;
import com.sun.jna.ptr.LongByReference;
import com.sun.jna.ptr.NativeLongByReference;

import java.io.Closeable;
import java.util.Arrays;
import java.util.List;

public class NativeItem extends Structure implements Closeable {
    public static class ByReference extends NativeItem implements Structure.ByReference {
    }

    public static class ByValue extends NativeItem implements Structure.ByValue {
    }

    public String uuid;
    public String itemName;
    public NativeLongByReference dueDate;
    public NativeLongByReference completionDate;

    @Override
    protected List<String> getFieldOrder() {
        return Arrays.asList("uuid", "itemName", "dueDate", "completionDate");
    }

    @Override
    public void close() {
        Log.i("NativeItem", "close");
        JNA.INSTANCE.item_c_destroy(this.getPointer());
    }
}
