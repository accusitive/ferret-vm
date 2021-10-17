echo "Building java" &&
    mkdir jbuild
javac Main.java -d jbuild &&
    echo "Building STD" &&
    clang -S -O3 -emit-llvm std.c &&
    echo "building Rust" &&
    cargo run &&
    echo "Building binary" &&
    gcc std.c out.o &&
    echo "Running binary" &&
    ./a.out
