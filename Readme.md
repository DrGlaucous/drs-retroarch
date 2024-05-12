# d-rs retroarch

**What is this?**

This repo is a "fork" of the [d-rs engine](https://github.com/doukutsu-rs/doukutsu-rs), (again... what is this, like the third repo I have dedicated to d-rs?!) for the purpose of porting it to the libretro environment. I have my changes in this dedicated repo instead of the [doukutsu-rs-nm](https://github.com/DrGlaucous/doukutsu-rs-nm) one because the workspace is very different, and switching between branches in the other repo really screws with my filesystem, so it was easier (for me) to just put everything in here.

### Feature Checklist
- [x] Screen drawing (openGL)
- [ ] Screen drawing (openGLES)
- [ ] Screen drawing (software?)
- [ ] Screen rescaling
- [x] Game timing
- [ ] Audio
- [ ] V-Sync support
- [ ] Core restarting
- [ ] Input
- [ ] Setting sync and adjustments
