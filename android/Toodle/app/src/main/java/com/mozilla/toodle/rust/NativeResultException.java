package com.mozilla.toodle.rust;

/**
 * Created by grisha on 12/13/17.
 */

public class NativeResultException extends Exception {
    private static final long serialVersionUID = 6502555018889722141L;

    NativeResultException(String message) {
        super(message);
    }
}
