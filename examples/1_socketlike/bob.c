#include <stdio.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() {

	// bob hard-codes his protocol.
	char* pdl =
	"primitive forward(in i, out o) {"
	"	while(true) synchronous {"
	"		put(o, get(i));"
	"	}"
	"}"
	;
	
	// setup a connector with one incoming network channel.
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "forward"));
	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_native(c, 1));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));
	
	// receive a message and print it to stdout three times
	int i;
	for (i = 0; i < 3; i++) {
		check("get ", connector_get(c, 0));
		check("sync", connector_sync(c, 1000));

		int msg_len;
		const unsigned char * msg;
		check("read", connector_gotten(c, 0, &msg, &msg_len));

		printf("Received one message `%s`!\n", msg);
	}

	// cleanup
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	return 0;
}