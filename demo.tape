# VHS Script for creating demo image
# https://github.com/charmbracelet/vhs
#
# Run:
# vhs demo.tape
# gifsicle --colors=255 demo.gif -o demo1.gif
# gifsicle --unoptimize -O3 --colors 128 demo1.gif "#16-" > screenshot.gif

Output demo.gif

Set Shell "bash"
Set FontSize 12
Set Width 640
Set Height 480

Type "cargo run" Enter

Sleep 3s
#Type "s"
#Sleep 3s
Down
Sleep 0.5s
Down
Sleep 0.5s
Down
Sleep 0.5s
Type "?"
Sleep 3s
Escape
Sleep 0.5s
Up
Sleep 0.5s
Up
Sleep 0.5s
Up
Sleep 1s
#Sleep 2s
#Type "s"
Sleep 3s
