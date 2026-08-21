# How to run BDGM media

To run a BDGM disc, you'll need a BDGM player installed. A very simple one is `bdgm-play` which we're going to use in these instructions.

## Windows
To install `bdgm-play`, we're going to need Rust and VS Build Tools.
Download rustup-init.exe for your architecture from: https://rust-lang.org/learn/get-started/ and run it. It should also install VS Build Tools.
After everything is installed, run `cargo install bdgm-play` in PowerShell to build and install `bdgm-play`.

After inserting a BDGM disc, an error may be shown that the disc could not be read. **This is fine**, because Windows decided to only support a small set of UDF discs.
Luckily, `bdgm-play` can read BDGM discs directly without needing Windows to parse the file system.  
To run a BDGM disc, check the disc drive's letter in This PC and run:
```
bdgm-play --raw-disc \\.\L:
```
***Replace `L` with your disc drive's letter.***

`bdgm-play` will copy data from the disc and run the game.

## Linux
On Linux, this process is easier because the kernel can actually mount BDGM discs.

To install `bdgm-play`, you need to install `cargo` from your package manager or using rustup; see https://rust-lang.org/learn/get-started/.
Then, run `cargo install bdgm-play`. Make sure `~/.cargo/bin` is in your `PATH`.

First, if your DE hasn't done this already, mount the disc to any readable directory. Then, run:
```bdgm-play /path/to/directory```

The path should not include the BDGM directory that's part of the disc.
