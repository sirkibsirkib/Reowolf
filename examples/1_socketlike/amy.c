#include <stdio.h>
#include <string.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() {
	
	// amy hard-codes her protocol.
	char* pdl =
	"primitive forward(in i, out o) {"
	"	while(true) synchronous {"
	"		put(o, get(i));"
	"	}"
	"}"
	;
	
	// fill a buffer with the user's message to send.
	char msg_buf[128];
	memset(msg_buf, 0, 128);
	printf("input a message to send:");
	check("fgets", fgets(msg_buf, 128-1, stdin) == NULL);
	int msg_len = strlen(msg_buf);
	msg_buf[msg_len-1] = 0;
	printf("will send msg `%s`\n", msg_buf);
	
	// create a connector with one outgoing network channel.
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "forward"));
	check("bind 0 ", connector_bind_native(c, 0));
	check("bind 1 ", connector_bind_passive(c, 1, "127.0.0.1:7000"));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));
	
	// send the user-provided message three times
	int i;
	for (i = 0; i < 3; i++) {
		check("put ", connector_put(c, 0, msg_buf, msg_len));
		check("sync", connector_sync(c, 1000));
		printf("Sent one message!\n");
	}
	
	// clean up
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	return 0;
}