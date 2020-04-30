#include <stdio.h>
#include <string.h>
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
	
	// create a connector with one outgoing network channel.
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "forward"));
	check("bind 0 ", connector_bind_native(c, 0));
	check("bind 1 ", connector_bind_passive(c, 1, "127.0.0.1:7000"));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));
	
	// send "hello" message three times
	int i;
	for (i = 0; i < 3; i++) {
		check("put ", connector_put(c, 0, "hello", 5));
		check("sync", connector_sync(c, 1000));
		printf("Sent one message!\n");
	}
	
	printf("cleaning up\n");
	connector_destroy(c);
	return 0;
}