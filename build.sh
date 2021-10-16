echo "Building java"
javac Main.java
echo "Building STD"
clang -S -emit-llvm std.c
echo "building Rust"
cargo run
echo "Building binary"
gcc std.c out.o
echo "Running binary"
./a.out