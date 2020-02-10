#include <stdio.h>
#include "../../reowolf.h"
#include "../check.c"

int main() {
	
	char* pdl ="\
	primitive forward(in i, out o) {\
		while(true) synchronous {\
			put(o, get(i));\
		}\
	}";
	
	// BOB
	Connector* c = connector_new();
	check("config ", connector_configure(c, pdl, "forward"));
	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_native(c, 1));
	check("connect", connector_connect(c, 10000));
	
	int i;
	for (i = 0; i < 3; i++) {
		check("get ", connector_get(c, 0));
		check("sync", connector_sync(c, 10000));

		int msg_len;
		const unsigned char * msg;
		check("read", connector_gotten(c, 0, &msg, &msg_len));

		printf("received: `%s`\n", msg);
	}
	
	printf("OK\n");
	connector_destroy(c);
	return 0;
}