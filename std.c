#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
enum StackObjectType { Int, Float };
union Stackobject {
  int i;
  float f;
};
struct Stack {
  union Stackobject data[1024];
  // enum StackObjectType types[1024];
  int head;
};
int stack_peeki(struct Stack *stack) { return stack->data[stack->head].i; }
int stack_peekf(struct Stack *stack) { return stack->data[stack->head].f; }

void stack_pushi(struct Stack *stack, int value) {
  // printf("Push %i\n", value);
  stack->data[++stack->head].i = value;
}
int stack_popi(struct Stack *stack) {
  int c = stack->data[stack->head--].i;
  if (stack->head < 0) {
    // printf("FATAL! Stack is less than 0! (%i)", stack->top);
    exit(1);
  }
  return c;
}

void stack_pushf(struct Stack *stack, float value) {
  printf("Push %f\n", value);
  stack->data[++stack->head].f = value;
}
float stack_popf(struct Stack *stack) {
  float c = stack->data[stack->head--].f;
  if (stack->head < 0) {
    // printf("FATAL! Stack is less than 0! (%i)", stack->top);
    exit(1);
  }
  return c;
}

struct Stack *stack_new() {
  struct Stack *s = malloc(1025);
  return s;
}
// static struct Stack globalStack = stack_new();

struct VariableStore {
  int data[1024];
};
void varstore_set(struct VariableStore *store, int index, int data) {
  store->data[index] = data;
}
struct VariableStore *varstore_new() {
  struct VariableStore *vs = malloc(1024);
  return vs;
}
int varstore_get(struct VariableStore *store, int index) {
  return store->data[index];
}
extern int add(int a, int b);
extern int testMethods(int a, int j);
extern int testMethods2(int a, int j);
extern int fib(int n);
extern float addf(float,float);
// int main() {

// printf("Doing le fibbonacci %i\n", fib(1));
// printf("Doing le fibbonacci %i\n", fib(2));
// printf("Doing le fibbonacci %i\n", fib(3));
// printf("Doing le fibbonacci %i\n", fib(8));
// printf("The output of add is (expect 6900) %i\n", add(1, 1));
// printf("The output of add is (expect 5000) %i\n", add(1, 2));
// printf("The output of add is (expect 1000) %i\n", add(5, 5));
// printf("The output of add is (expect 500) %i\n", testMethods(250, 999));
// printf("The output of add is (expect 500) %i\n", testMethods2(250));

// }
int main() {
  printf("Fadd(2.5f, 2.5f) = %f", addf(2.5f, 2.5f));
}
void println(int data) { printf("Println!! %i\n", data); }
// int main() {
//   struct Stack stack = {};
//   stack_pushi(&stack, 3);
//   assert(stack_popi(&stack) == 3);

//   stack_pushi(&stack, 512);
//   assert(stack_peeki(&stack) == 512);

//   stack_pushi(&stack, 10);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);

//   stack_pushi(&stack, 20);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);

//   stack_pushi(&stack, 30);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);

//   stack_pushi(&stack, 40);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);

//   assert(stack_popi(&stack) == 40);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);
//   assert(stack_popi(&stack) == 30);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);

//   assert(stack_popi(&stack) == 20);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);

//   assert(stack_popi(&stack) == 10);
//   printf("Peeked: %i Top: %i\n", stack_peeki(&stack), stack.head);

//   stack_pushf(&stack, 2.5f);
//   float f = stack_popf(&stack);
//   printf("Float %f", f);
// }
