#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
struct Stack {
  int data[1024];
  int top;
};
void stack_push(struct Stack *stack, int value) {
  // printf("Push %i\n", value);
  stack->data[++stack->top] = value;
}
int stack_peek(struct Stack *stack) { return stack->data[stack->top]; }
int stack_pop(struct Stack *stack) {
  // printf("Pop\n");

  int c = stack->data[stack->top--];
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

int main() {
  printf("The output of add is %i\n", add(1, 1));
  printf("The output of add is %i\n", add(1, 2));
}
void println(int data) { printf("Println!! %i\n", data); }
// int main() {
//   struct Stack stack = {};
//   stack_push(&stack, 3);
//   assert(stack_pop(&stack) == 3);

//   stack_push(&stack, 512);
//   assert(stack_peek(&stack) == 512);

//   stack_push(&stack, 10);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);

//   stack_push(&stack, 20);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);

//   stack_push(&stack, 30);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);

//   stack_push(&stack, 40);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);

//   assert(stack_pop(&stack) == 40);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);
//   assert(stack_pop(&stack) == 30);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);

//   assert(stack_pop(&stack) == 20);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);

//   assert(stack_pop(&stack) == 10);
//   printf("Peeked: %i Top: %i\n", stack_peek(&stack), stack.top);
// }
