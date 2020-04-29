#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() {
	char * pdl = buffer_pdl("eg_protocols.pdl");
	
	Connector* c = connector_new();
	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "sync"));
	check("bind 0 ", connector_bind_native(c, 0));
	check("bind 1 ", connector_bind_passive(c, 1, "127.0.0.1:7000"));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));

	// amy offers a message to her peer.
	// the message is the number of messages the peer previously received.

	int send_next = 0;
	char msg_buf[32];
	int code;
	int i;
	for (i = 0; 1; i++) {
		itoa(send_next, msg_buf, 10);
		printf("\nround %d. Will send msg `%s` next", i, msg_buf);
		
		// option (a): no messages sent
		check("next_batch ", connector_next_batch(c));
		
		// option (b): one message sent
		check("put ", connector_put(c, 0, msg_buf, strlen(msg_buf) + 1));
		code = connector_sync(c, 3000);
		
		// reflect on the outcome of the exchange
		if (code == 0) printf("Sent no message!");
		else if (code == 1) {
			printf("Sent message `%s`!", msg_buf);
			send_next++;
		} else {
			printf(
				"Connector error! %d (%s)\nBreaking loop!\n",
				code, connector_error_peek()
			);
			break;
		}
	}
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	free(pdl);
	return 0;
}