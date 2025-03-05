# Chip-8 Emulator in Rust
Yet another Chip-8 emulator! This was a fun intro project to learning Rust, and something I highly recommend for anyone looking to pick up a new programming language. Inspiration and direction for this project came from [this tutorial](https://tobiasvl.github.io/blog/write-a-chip-8-emulator/). Credit to [this book](https://github.com/aquova/chip8-book) for general project setup and structure.

## Requirements

- Rust (latest stable version)

## Installation and Usage

 Clone the repository, and then run the following command in the ```emu/``` directory of the project. Add the --legacy flag if the game doesn't run as expected.
 
```sh
cargo run -- ROM_NAME [--legacy]
```

To add additional games and programs, drop the ROMs into the folder ```roms/```

## Future Changes

- Event based input handling to hopefully improve responsiveness
- Support for more ambiguous instructions
- ~~Audio support~~

## License

[GPL 3.0](https://choosealicense.com/licenses/gpl-3.0/)