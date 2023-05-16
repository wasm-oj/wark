#include <stdio.h>
#include <stdlib.h>
#define KB 1024ULL
#define MB 1024 * KB

size_t linear_search_memory_limit(size_t max, size_t min);

int main() {
  size_t limit = linear_search_memory_limit(4096, 0);

  printf("Memory limit: %zu MB\n", limit);

  return 0;
}

size_t linear_search_memory_limit(size_t max, size_t min) {
  char *mem = NULL;

  for (size_t i = min; i < max; i++) {
    mem = malloc(i * MB);

    if (mem == NULL) {
      printf("Could not allocate %zu MB of memory.\n", i);
      return i - 1;
    } else {
      printf("Successfully allocated %zu MB of memory. [%p ~ %p]\n", i, mem,
             mem + (i * MB) - 1);

      free(mem);
      mem = NULL;
    }
  }

  return max;
}
