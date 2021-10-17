/**
 * Main
 */
public class Main {
    native int println(int data);

    public int add(int left, int right) {
        if (left == right) {
            if (left == 1) {
                return 6900;
            }
            return 1000;
        } else {
            return 5000;

        }
    }

    public int testMethods(int i, int j) {
        return testMethods2(i, j);
        // return i;
    }

    public int testMethods2(int i, int j) {
        println(i);
        println(j);

        return i + j;
    }

    public int fib(int n) {
        if (n < 2) {
            println(n);
            return n;
        }
        return fib(n - 1) + fib(n - 2);
    }
    // public int fib(int n) {
    // if(n < 5) {
    // return n;
    // }
    // return 10000;
    // }
}
