# d-rs retroarch

**What is this?**

This repo is a "fork" of the [d-rs engine](https://github.com/doukutsu-rs/doukutsu-rs), (again... what is this, like the third repo I have dedicated to d-rs?!) for the purpose of porting it to the libretro environment. I have my changes in this dedicated repo instead of the [doukutsu-rs-nm](https://github.com/DrGlaucous/doukutsu-rs-nm) one because the workspace is very different, and switching between branches in the other repo really screws with my filesystem, so it was easier (for me) to just put everything in here.

### Feature Checklist
- [x] Screen drawing (openGL)
- [X] Screen drawing (openGLES)
- [ ] Screen drawing (software?)
- [x] Screen rescaling *(see core display settings)*
- [x] Game timing
- [x] Audio
- [ ] V-Sync support
- [x] Core restarting
- [ ] Input (Keyboard) *(functions disabled because the keyboard can be mapped to the virualPad)*
- [x] Input (Gamepad)
- [x] Core display settings (aspect ratio, scale, etc.)
- [x] Filesystem
- [ ] Platforms
  - [x] Windows
  - [X] Linux
  - [ ] Mac OS
  - [x] Android
  - [ ] iOS


### Personal notes
<details>
	<summary>Notes</summary>
	To compile retroarch to use openglES, use:</br>
	<code>
  ./configure --disable-videocore --disable-opengl1 --enable-opengles --enable-opengles3 --enable-opengles3_1
  </code>
  
  </br>
   then</br>
	<code>make</code>

  to compile on mac for iOS, use
  `cargo lipo --release`

  then use codesign to give it an ad-hoc signature so it will run in retroarch:
  `codesign -s - drsretroarch.dylib`

  check sign status with
  `codesign -d -v drsretroarch.dylib`


</details>









