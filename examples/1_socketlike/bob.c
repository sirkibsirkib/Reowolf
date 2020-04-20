#include <stdio.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() { // BOB!
	
	char* pdl =
	"primitive forward(in i, out o) {"
	"	while(true) synchronous {"
	"		put(o, get(i));"
	"	}"
	"}"
	;
	
	// BOB
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "forward"));
	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_native(c, 1));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));
	
	int i;
	for (i = 0; i < 3; i++) {
		check("get ", connector_get(c, 0));
		check("sync", connector_sync(c, 1000));

		int msg_len;
		const unsigned char * msg;
		check("read", connector_gotten(c, 0, &msg, &msg_len));

		printf("Received one message `%s`!\n", msg);
	}
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	return 0;
}