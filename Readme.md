![doukutsu-rs-libretro](./Core/Development/Media/DK-Rust-Mascot-CRAB-libretro.png)
# d-rs retroarch (yet another Cave Story port)

**What is this?**

This repo is a "fork" of the [d-rs engine](https://github.com/doukutsu-rs/doukutsu-rs), (again... what's this, like the third repo I have dedicated to d-rs?!) for the purpose of porting it to the libretro environment. I have my changes in this dedicated repo instead of the [doukutsu-rs-nm](https://github.com/DrGlaucous/doukutsu-rs-nm) one because the workspace is very different, and switching between branches in the other repo really screws with my filesystem, so it was easier (for me) to just put everything in here.

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

Since this port isn't currently part of libretro's upstream build system, some extra steps need to be taken in order to use it with Retroarch. 

Before loading in the core, the `drsretroarch.info` file must be placed with the other info files in Retroarch's documents directory. This is because even though the core will work just fine without it, the Retroarch UI requires the info file to understand how to load files to the core. Without it, the frontend will refuse to start the game.

To use the UI with the nintendo switch port of Cave Story, you need to put a dummy target in next to the `data` directory. This is because the UI requires *some* file to "load in", regardless if the core actually uses it or not. An empty text file named `Target.exe` will work just fine for this purpose (the only important part is the `.exe` extension).

If the command line is used to load in the core instead, both of these prerequisites can be ignored.

### Building

In build this core, rustc version `1.72.1` must be used. This is because of a glitch with imGUI and later versions of Rust. The upstream version of d-rs has since fixed this, but this version is currently derived from before the upstream fix.

Simply run the `cargo build` command from within the drsretroarch subdirectory to build the core for the parent system. To build it for other systems, the process is no different than building a generic library. (For instance, for android, you'd use [cargo-ndk](https://github.com/bbqsrc/cargo-ndk), or for iOS, [cargo lipo](https://github.com/TimNN/cargo-lipo).)

A pre-built example for Windows can be found in the `Development` folder.

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

---

to compile on mac for iOS, use
`cargo lipo --release`

then use codesign to give it an ad-hoc signature so it will run in retroarch:
`codesign -s - drsretroarch.dylib`

check sign status with
`codesign -d -v drsretroarch.dylib`

For convenient developing, just use the `make ios` command in the drsretroarch subdirectory. It will sign and rename the output dll for you. *(I need to add makefile support for the other systems as well, since the linux-based distros automatically add 'lib' to the front of the output.)*

The ad-hoc signed files work with both the sideloaded and appstore versions of retroarch, but it's impossible to put the core in the framework directory with the rest without jailbreaking the phone, so at that point, it's just easier to use the sideloaded version. *(also moving the core directory doesn't work because the apple sandbox forbids loading outside frameworks)*

</details>









