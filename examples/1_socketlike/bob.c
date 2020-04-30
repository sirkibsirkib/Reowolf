#include <stdio.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() {

	// Protocol is hard-coded.
	char* pdl =
	"primitive forward(in i, out o) {"
	"  while(true) synchronous {     "
	"    put(o, get(i));             "
	"  }                             "
	"}                               ";
	
	// setup a connector with one incoming network channel.
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config  ", connector_configure(c, pdl, "forward"));
	check("bind 0  ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1  ", connector_bind_native(c, 1));
	check("connect ", connector_connect(c, 5000));
	
	// receive a message and print it to stdout three times
	int i, msg_len;
	const unsigned char * msg;
	for (i = 0; i < 3; i++) {
		check("get ", connector_get(c, 0));
		check("sync", connector_sync(c, 1000));
		check("read", connector_gotten(c, 0, &msg, &msg_len));
		printf("Received one message `%.*s`!\n", msg_len, msg);
	}
	
	printf("cleaning up\n");
	connector_destroy(c);
	return 0;
}