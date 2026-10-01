# Regenerate the demo GIF with VHS (https://github.com/charmbracelet/vhs).
#
# Run from this directory, with the plugin already installed in Nushell:
#     cd nu_plugin_charton/assets && vhs demo.tape
#
# The recording forces the half-block renderer, because terminal recorders
# cannot capture the Kitty/iTerm2 inline image escape sequences. The committed
# nushell-demo.gif is the crisp image-protocol version instead.

Output nushell-demo.gif

Set Shell "nu"
Set Width 1280
Set Height 800
Set FontSize 20
Set Padding 20
Set TypingSpeed 55ms

Type "open demo.csv | charton -g point -x x -y y -c group --size-by size --theme dark --inline-style halfblock"
Enter
Sleep 4s
