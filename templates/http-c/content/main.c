#include <stdio.h>
#include <stdlib.h>

int main() {
    printf("content-type: text/plain\n\n");
    printf("Hello from %s\n", getenv("SERVER_SOFTWARE"));
}
