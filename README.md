# bitOS
This is my (currently) non-functioning operating system kernel based on 
[the tutorial by Philipp Oppermann](https://os.phil-opp.com/).

## Building and running the kernel

### Preriquisites
These programs and tools should be installed before trying to build the kernel.
Because the installation of these tools and programs is platform-dependent, 
I will just link their websites, which will explain how to install them.

* [Any GNU/Linux system](https://en.wikipedia.org/wiki/List_of_Linux_distributions)
* [The Rust Programming Language](https://www.rust-lang.org/learn/get-started)
* [QEMU](https://www.qemu.org/download/)

### Installation guide
To clone, build, and run the kernel, you need to run the following commands: 
```bash
# clone project
git clone https://github.com/JM4ier/bitOS.git
cd bitOS/

# enable execution of the build setup script
chmod u+x ./setup-build

# run the build setup script
./setup-build

# build the kernel
cargo xrun
```
