#include <stdio.h>
#include <stdlib.h>
#include <time.h>

int main() {
  srand(time(NULL));
  for (int i = 0; i < 10; i++) {
    int random = rand();
    for (int j = 0; j < sizeof(int); j++) {
      printf("%02x", (random >> (j * 8)) & 0xff);
    }
  }
  return 0;
}
