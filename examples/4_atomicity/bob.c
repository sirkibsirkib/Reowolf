#include <stdio.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() { // BOB!
	char * pdl = buffer_pdl("eg_protocols.pdl");
	Connector* c = connector_new();

	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "sync_two"));

	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_active(c, 1, "127.0.0.1:7001"));
	check("bind 2 ", connector_bind_native(c, 2));
	check("bind 3 ", connector_bind_native(c, 3));

	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));

	int msg_len;
	const unsigned char * msg;

	int i;
	
	// rounds 0..=2: get both messages
	for (i = 0; i <= 2; i++) {
		printf("\nround %d\n", i);
		
		check("get ", connector_get(c, 0));
		check("get ", connector_get(c, 1));
		check("sync", connector_sync(c, 1000));
		
		check("read one", connector_gotten(c, 0, &msg, &msg_len));
		printf("Got message one: `%.*s`\n", msg_len, msg);
		
		check("read two", connector_gotten(c, 1, &msg, &msg_len));
		printf("Got message two: `%.*s`\n", msg_len, msg);
	}
	// rounds 3..=5: get neither message
	for (i = 3; i <= 5; i++) {
		printf("\nround %d\n", i);
		
		//check("get ", connector_get(c, 0));
		//check("get ", connector_get(c, 1));
		check("sync", connector_sync(c, 3000));
		
		//check("read one", connector_gotten(c, 0, &msg, &msg_len));
		//printf("Got message one: `%.*s`\n", msg_len, msg);
		
		//check("read two", connector_gotten(c, 1, &msg, &msg_len));
		//printf("Got message two: `%.*s`\n", msg_len, msg);
	}
	// round 6: attempt to get just one message
	for (i = 6; i <= 6; i++) {
		printf("\nround %d\n", i);
		
		check("get ", connector_get(c, 0));
		//check("get ", connector_get(c, 1));
		check("sync", connector_sync(c, 3000));
		
		//check("read one", connector_gotten(c, 0, &msg, &msg_len));
		//printf("Got message one: `%.*s`\n", msg_len, msg);
		
		check("read two", connector_gotten(c, 1, &msg, &msg_len));
		printf("Got message two: `%.*s`\n", msg_len, msg);
	}
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	free(pdl);
	return 0;
}