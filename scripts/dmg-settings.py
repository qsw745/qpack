# -*- coding: utf-8 -*-
import os

ROOT_DIR = os.getcwd()
APP_PATH = os.path.join(ROOT_DIR, "src-tauri/target/release/bundle/macos/QPack.app")
BG_PATH = os.path.join(ROOT_DIR, "src-tauri/icons/dmg-background.tiff")
ICON_PATH = os.path.join(ROOT_DIR, "src-tauri/icons/icon.icns")

# 磁盘镜像基本设置
format = "UDZO"
compression_level = 9
size = None

# 打包内容：QPack.app + 指向 /Applications 的软链接
files = [APP_PATH]
symlinks = {"Applications": "/Applications"}

# 卷标图标
icon = ICON_PATH

# 窗口与视图布局（严格匹配 660x420 背景图）
window_rect = ((200, 140), (660, 420))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
arrange_by = None
grid_offset = (0, 0)
grid_spacing = 100
scroll_position = (0, 0)
label_pos = "bottom"
text_size = 13
icon_size = 108

# HiDPI 背景图（1x 660x420 + 2x 1320x840）
background = BG_PATH

# 图标精确落位（对准背景图中的左右磨砂托盘中心）
icon_locations = {
    "QPack.app": (180, 212),
    "Applications": (480, 212),
}
