# 托盘
点击：切换idle
  dbus
## 菜单
- screen-off
- toggle
- status
- toggle tomato
- quit
# 命令行
-o: 关闭屏幕，退出程序，跳过其他功能。
-i: 启动并保持常亮
# 启动
护眼提醒 番茄钟

# 流程
if -o:
  close screen
  exit
end

创建托盘

if -i:
  add inhibit
else:
  add play track thread
end

add thread: care eye notice

####################
add_inhi
  stop play track

remove_inhi
  start play track

##################
dep:
  clap
  dbus
