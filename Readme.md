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
- [ ] Input (touch) *(carryover from the android port where certain items like menus can be interracted with)*
- [x] Input (Gamepad)
- [x] Core display settings (aspect ratio, scale, etc.)
- [x] Filesystem
- [ ] Platforms
  - [x] Windows
  - [X] Linux
  - [ ] Mac OS *(openGL shaders are currently broken here)*
  - [x] Android
  - [X] iOS


### Use
It its current state, d-rs runs on 4/5 "big" platforms. *(mac OS was tried, but the compatibility context for harware rendering was broken. Shaders for the backend would not compile, and if the openGL context were set to a version where they'd work, the **frontend's** shaders would break. This doesn't matter too much since d-rs already has a native mac port)*



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









