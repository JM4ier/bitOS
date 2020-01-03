# bitOS
This is my (currently) non-functioning operating system kernel based on 
[the tutorial by Philipp Oppermann](https://os.phil-opp.com/).

## Building and running the kernel

### Preriquisites
* [The Rust Programming Language](https://www.rust-lang.org/learn/get-started)
* [QEMU](https://www.qemu.org/download/)

### Installation guide
* Clone the repository and change directory

  `git clone https://github.com/JM4ier/bitOS.git`
  
  `cd bitOS`
    
* Run the build tool installation script

  `chmod u+x ./setup-build`
  
  `./setup-build`
  
* Run the kernel

  `cargo xrun`
   
