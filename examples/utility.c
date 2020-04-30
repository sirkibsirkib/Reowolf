#include <stdio.h>
#include <stdlib.h>
#include <errno.h>

void check(const char* phase, int code) {
	if (code < 0) {
		printf("ERR %d in phase `%s`. Err was `%s`\nEXITING!\n",
			code, phase, connector_error_peek());
		exit(1);
	}
}

// allocates a buffer!
char * buffer_pdl(char * filename) {
	FILE *f = fopen(filename, "rb");
	if (f == NULL) {
		printf("Opening pdl file returned errno %d!\n", errno);
		exit(1);
	}
	fseek(f, 0, SEEK_END);
	long fsize = ftell(f);
	fseek(f, 0, SEEK_SET);
	char *pdl = malloc(fsize + 1);
	fread(pdl, 1, fsize, f);
	fclose(f);
	pdl[fsize] = 0;
	return pdl;
}