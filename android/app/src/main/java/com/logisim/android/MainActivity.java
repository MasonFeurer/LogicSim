package com.logisim.android;

import androidx.core.view.WindowCompat;
import androidx.core.view.ViewCompat;
import androidx.core.view.DisplayCutoutCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;
import androidx.core.graphics.Insets;

import com.google.androidgamesdk.GameActivity;

import android.os.Build.VERSION;
import android.os.Build.VERSION_CODES;
import android.os.Bundle;
import android.view.View;
import android.view.WindowManager;

public class MainActivity extends GameActivity {
    static {
        System.loadLibrary("main");
    }

    native void onDisplayInsets(int[] cutouts);

    private void createInsetsListener() {
        // Listener for display insets (cutouts) to pass values into native code.
        View content = getWindow().getDecorView().findViewById(android.R.id.content);
        ViewCompat.setOnApplyWindowInsetsListener(content, (v, insets) -> {
            DisplayCutoutCompat dc = insets.getDisplayCutout();
            int cutoutTop = 0;
            int cutoutRight = 0;
            int cutoutBottom = 0;
            int cutoutLeft = 0;
            if (dc != null) {
                cutoutTop = dc.getSafeInsetTop();
                cutoutRight = dc.getSafeInsetRight();
                cutoutBottom = dc.getSafeInsetBottom();
                cutoutLeft = dc.getSafeInsetLeft();
            }
            Insets systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars());

            int[] values = new int[]{0, 0, 0, 0};
            values[0] = Integer.max(cutoutTop, systemBars.top);
            values[1] = Integer.max(cutoutRight, systemBars.right);
            values[2] = Integer.max(cutoutBottom, systemBars.bottom);
            values[3] = Integer.max(cutoutLeft, systemBars.left);
            onDisplayInsets(values);
            return insets;
        });
    }

    private void hideSystemUI() {
        // This will put the game behind any cutouts and waterfalls on devices which have
        // them, so the corresponding insets will be non-zero.
        if (VERSION.SDK_INT >= VERSION_CODES.P) {
            getWindow().getAttributes().layoutInDisplayCutoutMode
                    = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        }
        // From API 30 onwards, this is the recommended way to hide the system UI, rather than
        // using View.setSystemUiVisibility.
        View decorView = getWindow().getDecorView();
        WindowInsetsControllerCompat controller = new WindowInsetsControllerCompat(getWindow(),
                decorView);
        controller.hide(WindowInsetsCompat.Type.systemBars());
        controller.hide(WindowInsetsCompat.Type.displayCutout());
        controller.setSystemBarsBehavior(
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // When true, the app will fit inside any system UI windows.
        // When false, we render behind any system UI windows.
        WindowCompat.setDecorFitsSystemWindows(getWindow(), true);
        hideSystemUI();
        createInsetsListener();
        super.onCreate(savedInstanceState);
    }

    protected void onResume() {
        super.onResume();
        hideSystemUI();
    }
}