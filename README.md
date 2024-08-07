# CS2 Schema Offset Lookup Tool
This project is a Rust-based tool that provides a convenient interface for looking up fields of game structures using their memory offsets.

## Features
- Look up fields by offset in a user-friendly manner
- Platform-independent, unlike source2gen
- Lightweight and efficient (50ms startup time, negligible lookup time)
- Convenient `.env` configuration
- Easy to read color-coded output

## Getting Started
Before using this, ensure you have the required Rust environment to compile and run Rust applications. If you're new to Rust, you can get started [here](https://www.rust-lang.org/learn/get-started).

### Prerequisites
- Rustc and Cargo
- A JSON file containing the schema

### Setup Environment
1. Clone this repository to your local machine.
2. Copy `.env.example` to `.env`.
```bash
cp .env.example .env
```
3. Open the `.env` file in a text editor and specify the path to your `client.hpp` file.
```bash
SCHEMA_JSON="/path/to/your/schema.json"
```
4. Save `.env` with the updated path.

## Compilation
After setting up your environment, you can compile the project using Cargo:
```bash
cargo build --release
```
The compiled binary will be located in the `target/release` directory.

## Usage
Run the compiled binary from to start the application. The interface will prompt you to enter an offset value and find corresponding fields in the parsed SDK files.
```bash
./target/release/sdk-lookup
```
To exit, simply type `exit` and press enter, or send a SIGINT signal (Ctrl+C).

## Credits

- Mistral-medium for this README.

## License
This project is licensed under the GPL 3.0 License - see the LICENSE file for details.
