package com.mozilla.toodle.rust;

import android.util.Log;

import com.sun.jna.Structure;

import java.io.Closeable;
import java.util.Arrays;
import java.util.List;


public class NativeItemSet extends Structure implements Closeable {
    public static class ByReference extends NativeItemSet implements Structure.ByReference {
    }

    public static class ByValue extends NativeItemSet implements Structure.ByValue {
    }

    public NativeItem.ByReference items;
    public int numberOfItems;
    public int len;

    public List<NativeItem> getItems() {
        final NativeItem[] array = (NativeItem[]) items.toArray(numberOfItems);
        return Arrays.asList(array);
    }

    @Override
    protected List<String> getFieldOrder() {
        return Arrays.asList("items", "numberOfItems", "len");
    }

    @Override
    public void close() {
        Log.i("NativeItemSet", "close");
        final NativeItem[] nativeItems = (NativeItem[]) items.toArray(numberOfItems);
        for (NativeItem nativeItem : nativeItems) {
            nativeItem.close();
        }
    }
}
