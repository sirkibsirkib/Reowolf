#include <stdio.h>
#include <string.h>
#include "../../reowolf.h"

int main() {
	// ALICE
	
	char* pdl ="\
	primitive forward(in i, out o) {\
		while(true) synchronous {\
			put(o, get(i));\
		}\
	}";
	
	
	char msg_buf[128];
	memset(msg_buf, 0, 128);
	
	printf("input a message to send:");
	if (fgets(msg_buf, 128-1, stdin) == NULL) {
		printf("LINE READ BAD\n");
		return 1;
	}
	int msg_len = strlen(msg_buf);
	msg_buf[msg_len-1] = 0;
	printf("sending msg `%s`\n", msg_buf);
	
	Connector* c = connector_new();
	if (connector_configure(c, pdl, "forward")) {
		printf("CONFIG FAILED: %s\n", connector_error_peek());
		return 1;
	}
	if (connector_bind_native(c, 0)) {
		printf("BIND0 FAILED: %s\n", connector_error_peek());
		return 1;
	}
	if (connector_bind_passive(c, 1, "127.0.0.1:7000")) {
		printf("BIND1 FAILED: %s\n", connector_error_peek());
		return 1;
	}
	printf("connecting... \n");
	if (connector_connect(c, 10000)) {
		printf("CONNECT FAILED: %s\n", connector_error_peek());
		return 1;
	}
	
	int i;
	for (i = 0; i < 3; i++) {
		if (connector_put(c, 0, msg_buf, msg_len)) {
			printf("CONNECT PUT: %s\n", connector_error_peek());
			return 1;
		}
		if (connector_sync(c, 10000)) {
			printf("SYNC FAILED: %s\n", connector_error_peek());
			return 1;
		}
		printf("SEND OK\n");
	}
	
	printf("OK\n");
	connector_destroy(c);
	return 0;
}