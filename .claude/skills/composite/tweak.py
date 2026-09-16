# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "opencv-python", "pillow", "matplotlib", "PyQt6"]
# ///
"""
SUPER SIMPLE tweaker UI for composite settings.
Zoomed view of frame area. Before/after toggle. Direct save.
"""

import sys
import os

os.environ.setdefault('QT_QPA_PLATFORM', 'wayland')
import matplotlib
matplotlib.use('QtAgg')

import numpy as np
import cv2
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider, Button, CheckButtons
from pathlib import Path

from composite import (
    detect_green_mask,
    find_quadrilateral_corners,
    expand_corners_outward,
    warp_print_to_frame,
    extract_near_frame_region,
    reinhard_color_transfer,
    composite_with_feathering,
)


def tweak(template_path: str, print_path: str):
    template_path = Path(template_path)
    print_path_p = Path(print_path)

    # Load
    template = cv2.imread(str(template_path))
    print_img = cv2.imread(str(print_path))

    if template is None or print_img is None:
        print("Error loading images")
        sys.exit(1)

    output_size = (template.shape[1], template.shape[0])

    # State
    state = {
        'show_original': False,
        'full_view': False,
        'last_result': None,
        'crop': None,
    }

    def render(expand, erode, feather, brightness, strength, hue_min, hue_max, sat_min, val_min, color_margin, scale):
        # Re-detect mask with current HSV params
        mask = detect_green_mask(template, int(hue_min), int(hue_max), int(sat_min), int(val_min))
        base_corners = find_quadrilateral_corners(mask, expand_px=0)

        if base_corners is None:
            # Return template as-is if detection fails
            state['last_result'] = template.copy()
            return template.copy()

        # Update crop region based on current corners
        x_min, y_min = base_corners.min(axis=0).astype(int)
        x_max, y_max = base_corners.max(axis=0).astype(int)
        margin = 40
        state['crop'] = (
            max(0, y_min - margin),
            min(template.shape[0], y_max + margin),
            max(0, x_min - margin),
            min(template.shape[1], x_max + margin),
        )

        corners = expand_corners_outward(base_corners, expand) if expand > 0 else base_corners
        warped, warped_mask = warp_print_to_frame(print_img, corners, output_size, scale=scale)
        target_pixels = extract_near_frame_region(template, base_corners, mask, int(color_margin))
        harmonized = reinhard_color_transfer(warped, target_pixels, strength, brightness)
        result = composite_with_feathering(template, harmonized, warped_mask, feather, erode, green_mask=mask)
        state['last_result'] = result
        return result

    def get_display(result):
        """Get display image (cropped or full)."""
        img = template if state['show_original'] else result
        if state['full_view'] or state['crop'] is None:
            return cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
        else:
            crop = state['crop']
            cropped = img[crop[0]:crop[1], crop[2]:crop[3]]
            return cv2.cvtColor(cropped, cv2.COLOR_BGR2RGB)

    # Layout: main view at top, two columns of sliders below
    fig = plt.figure(figsize=(12, 10))
    fig.canvas.manager.set_window_title('Composite Tweaker')

    ax_img = plt.axes([0.05, 0.38, 0.9, 0.58])
    ax_img.set_title('Frame Detail', fontsize=10)
    ax_img.axis('off')

    initial = render(5, 3, 8, -20, 1.0, 35, 85, 40, 40, 50, 1.0)
    img_display = ax_img.imshow(get_display(initial))

    # Slider styling
    slider_color = '#4a90d9'
    slider_color_alt = '#d9904a'

    # Left column - Compositing (x: 0.08 to 0.42)
    left_x, left_w = 0.12, 0.33
    ax_expand  = plt.axes([left_x, 0.30, left_w, 0.018])
    ax_erode   = plt.axes([left_x, 0.26, left_w, 0.018])
    ax_feather = plt.axes([left_x, 0.22, left_w, 0.018])
    ax_bright  = plt.axes([left_x, 0.18, left_w, 0.018])
    ax_color   = plt.axes([left_x, 0.14, left_w, 0.018])
    ax_margin  = plt.axes([left_x, 0.10, left_w, 0.018])

    s_expand  = Slider(ax_expand, 'Expand', 0, 20, valinit=5, valstep=1, color=slider_color)
    s_erode   = Slider(ax_erode, 'Erode (neg=dilate)', -15, 15, valinit=3, valstep=1, color=slider_color)
    s_feather = Slider(ax_feather, 'Feather', 0, 30, valinit=8, valstep=1, color=slider_color)
    s_bright  = Slider(ax_bright, 'Brightness', -80, 40, valinit=-20, valstep=5, color=slider_color)
    s_color   = Slider(ax_color, 'Color Match', 0, 2, valinit=1.0, valstep=0.1, color=slider_color)
    s_margin  = Slider(ax_margin, 'Sample Margin', 10, 150, valinit=50, valstep=5, color=slider_color)

    # Right column - Green Detection + Scale (x: 0.58 to 0.92)
    right_x, right_w = 0.62, 0.33
    ax_hue_min = plt.axes([right_x, 0.30, right_w, 0.018])
    ax_hue_max = plt.axes([right_x, 0.26, right_w, 0.018])
    ax_sat_min = plt.axes([right_x, 0.22, right_w, 0.018])
    ax_val_min = plt.axes([right_x, 0.18, right_w, 0.018])
    ax_scale   = plt.axes([right_x, 0.14, right_w, 0.018])

    s_hue_min = Slider(ax_hue_min, 'Hue Min', 0, 90, valinit=35, valstep=1, color=slider_color_alt)
    s_hue_max = Slider(ax_hue_max, 'Hue Max', 45, 180, valinit=85, valstep=1, color=slider_color_alt)
    s_sat_min = Slider(ax_sat_min, 'Sat Min', 0, 255, valinit=40, valstep=5, color=slider_color_alt)
    s_val_min = Slider(ax_val_min, 'Val Min', 0, 255, valinit=40, valstep=5, color=slider_color_alt)
    s_scale   = Slider(ax_scale, 'Print Scale', 0.5, 2.0, valinit=1.0, valstep=0.05, color=slider_color_alt)

    # Column labels
    fig.text(0.28, 0.335, 'Compositing', ha='center', fontsize=9, fontweight='bold', color='#333')
    fig.text(0.78, 0.335, 'Green Detection', ha='center', fontsize=9, fontweight='bold', color='#333')

    # Hints
    ax_expand.set_title('← edges visible | covers frame →', fontsize=7, loc='right', color='gray')
    ax_bright.set_title('← darker | brighter →', fontsize=7, loc='right', color='gray')
    ax_hue_min.set_title('green hue range in HSV', fontsize=7, loc='right', color='gray')
    ax_scale.set_title('← smaller | larger/crop →', fontsize=7, loc='right', color='gray')

    def update(val=None):
        result = render(
            float(s_expand.val), int(s_erode.val), int(s_feather.val),
            float(s_bright.val), float(s_color.val),
            int(s_hue_min.val), int(s_hue_max.val), int(s_sat_min.val), int(s_val_min.val),
            int(s_margin.val), float(s_scale.val)
        )
        new_img = get_display(result)
        img_display.set_data(new_img)
        img_display.set_extent([0, new_img.shape[1], new_img.shape[0], 0])
        ax_img.set_xlim(0, new_img.shape[1])
        ax_img.set_ylim(new_img.shape[0], 0)
        fig.canvas.draw_idle()

    for s in [s_expand, s_erode, s_feather, s_bright, s_color, s_margin,
              s_hue_min, s_hue_max, s_sat_min, s_val_min, s_scale]:
        s.on_changed(update)

    # Checkboxes for view options
    ax_check = plt.axes([0.05, 0.02, 0.15, 0.06])
    check = CheckButtons(ax_check, ['Original', 'Full'], [False, False])

    def toggle_view(label):
        if label == 'Original':
            state['show_original'] = not state['show_original']
        elif label == 'Full':
            state['full_view'] = not state['full_view']
        new_img = get_display(state['last_result'])
        img_display.set_data(new_img)
        img_display.set_extent([0, new_img.shape[1], new_img.shape[0], 0])
        ax_img.set_xlim(0, new_img.shape[1])
        ax_img.set_ylim(new_img.shape[0], 0)
        fig.canvas.draw_idle()

    check.on_clicked(toggle_view)

    # Save button
    ax_save = plt.axes([0.7, 0.02, 0.12, 0.05])
    btn_save = Button(ax_save, 'SAVE', color='#2ecc71', hovercolor='#27ae60')

    def save(event):
        out_path = template_path.parent / f"{template_path.stem}_composite{template_path.suffix}"
        result_rgb = cv2.cvtColor(state['last_result'], cv2.COLOR_BGR2RGB)
        from PIL import Image
        Image.fromarray(result_rgb).save(out_path, quality=95)
        print(f"\n✓ Saved: {out_path}")
        print_command()

    btn_save.on_clicked(save)

    # Reset button
    ax_reset = plt.axes([0.84, 0.02, 0.12, 0.05])
    btn_reset = Button(ax_reset, 'Reset', color='#95a5a6', hovercolor='#7f8c8d')

    def reset(event):
        s_expand.set_val(5)
        s_erode.set_val(3)
        s_feather.set_val(8)
        s_bright.set_val(-20)
        s_color.set_val(1.0)
        s_margin.set_val(50)
        s_hue_min.set_val(35)
        s_hue_max.set_val(85)
        s_sat_min.set_val(40)
        s_val_min.set_val(40)
        s_scale.set_val(1.0)

    btn_reset.on_clicked(reset)

    def print_command():
        print("\n" + "─"*60)
        print("CLI command:")
        print("─"*60)
        cmd = f"uv run .claude/skills/composite/composite.py \\\n"
        cmd += f"  '{template_path}' \\\n"
        cmd += f"  '{print_path}' \\\n"
        cmd += f"  --expand {s_expand.val} --erode {int(s_erode.val)} --feather {int(s_feather.val)} \\\n"
        cmd += f"  --brightness {s_bright.val} --color-strength {s_color.val} \\\n"
        cmd += f"  --hue-min {int(s_hue_min.val)} --hue-max {int(s_hue_max.val)} \\\n"
        cmd += f"  --sat-min {int(s_sat_min.val)} --val-min {int(s_val_min.val)} \\\n"
        cmd += f"  --color-margin {int(s_margin.val)} --scale {s_scale.val}"
        print(cmd)
        print("─"*60)

    plt.show()
    print_command()


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: uv run tweak.py TEMPLATE PRINT")
        sys.exit(1)
    tweak(sys.argv[1], sys.argv[2])
