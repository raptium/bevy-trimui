# 在 TrimUI Brick Pro 上运行 Bevy

> **本项目全部由 AI 制作，使用风险及后果自负，不作任何担保。**
> 在法律允许的最大范围内，作者不承担责任，包括设备损坏、变砖或爆炸。
> 直白地说：机器爆炸了，也别找作者负责。详见[免责声明](DISCLAIMER.md)。

这是一套从 Boat 提取的独立平台工具：原生 Bevy / wgpu Vulkan 渲染、
`VK_KHR_display` 上屏、evdev 退出、厂商驱动兼容补丁和像素验收工具。
不需要游戏美术、字体、设备私有库或原项目 Git 历史。

已验证基线：**Brick Pro，固件 1.1.2，PowerVR GE8300，DDK 1.19@6345021，
Bevy 0.19.1 / wgpu 29.0.4**。2026-10-09 的实机动画完成多帧像素核对，
操作者确认移动正常、不闪并自动返回。不能据此宣称原版 Brick、其他固件、
3D 或任意 Bevy 插件均兼容。0.20 也有历史上屏结果，但本包构建入口固定在 0.19.1。

先读 [英文主文档](README.md) 中的环境、构建、SSH 设置与部署命令。
AI 从 [AGENTS.md](AGENTS.md) 开始，或使用
[trimui-brick-bevy Skill](skills/trimui-brick-bevy/SKILL.md)。

```sh
rustup target add aarch64-unknown-linux-gnu
python3 -m venv .venv
. .venv/bin/activate
python3 -m pip install -r requirements-dev.txt
python3 tools/device/build_bevy_spike.py
python3 tools/device/package_bevy_spike.py --seconds 30
# 配好自己的 ssh.local.sh 后安装
python3 tools/device/install_app.py \
  --package dist/Apps/boat-bevy-wgpu --app-name boat-bevy-wgpu
# 在掌机 Apps 里启动，等自动返回后收集
python3 tools/device/collect_bevy_spike.py
```

成功标志是 `BEVY_ANIMATION_PASSED`，包含二进制哈希、退出状态、多帧实际像素与
场景坐标核对。编译成功、进程退出 0、不断调用 present，都不能单独证明上屏正确。
通过 SSH 直接上屏会与 launcher 抢屏；肉眼验收必须从 Apps 启动。

文档按任务拆分：

- [设备研究](docs/device.md)：真实运行库、SDK 差异、Vulkan／DRM 判断边界。
- [接入自己的 Bevy 应用](docs/integration.md)：平台 crate、版本固定、surface 所有权、输入。
- [音频](docs/audio.md)：ALSA 独立配置、S16_LE／48 kHz、CPAL 局部补丁与听音验收。
- [故障排查](docs/troubleshooting.md)：黑图、闪屏、shader、缓存与 SSH。
- [验证记录](docs/evidence.md)：历史实机结果、未测范围及新机器的验收方法。
- [机器可读兼容记录](docs/compatibility.json)：供 AI 自动定位版本、结果和复现命令。

原创代码、文档和 Skill 采用 MIT；上游依赖保持原许可。固件快照由使用者自行采集，
不会随开源包分发。详见 [NOTICE](NOTICE.md)。
