#include <stdio.h>
#include "../../reowolf.h"

int main() {
	
	char* pdl ="\
	primitive forward(in i, out o) {\
		while(true) synchronous {\
			put(o, get(i));\
		}\
	}";
	
	// BOB
	Connector* c = connector_new();
	if (connector_configure(c, pdl, "forward")) {
		printf("CONFIG FAILED: %s\n", connector_error_peek());
		return 1;
	}
	if (connector_bind_active(c, 0, "127.0.0.1:7000")) {
		printf("BIND0 FAILED: %s\n", connector_error_peek());
		return 1;
	}
	if (connector_bind_native(c, 1)) {
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
		if (connector_get(c, 0)) {
			printf("CONNECT GET: %s\n", connector_error_peek());
			return 1;
		}
		if (connector_sync(c, 10000)) {
			printf("SYNC FAILED: %s\n", connector_error_peek());
			return 1;
		}
		int msg_len;
		const unsigned char * msg;
		if (connector_gotten(c, 0, &msg, &msg_len)) {
			printf("READ FAILED: %s\n", connector_error_peek());
			return 1;
		}
		printf("received: `%s`\n", msg);
	}
	
	printf("OK\n");
	connector_destroy(c);
	return 0;
}