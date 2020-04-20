#include <stdio.h>
#include <time.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() { // BOB!
	char * pdl = buffer_pdl("eg_protocols.pdl");
	Connector* c = connector_new();

	printf("configuring...\n");
	check("config ", connector_configure(c, pdl, "sync"));

	check("bind 0 ", connector_bind_active(c, 0, "127.0.0.1:7000"));
	check("bind 1 ", connector_bind_native(c, 1));

	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));

	int msg_len;
	const unsigned char * msg;

	int i;
	srand(time(NULL));
	for (i = 0; i < 10; i++) {
		printf("\nround %d...\n", i);
		int random = rand() % 2;
		if (random == 0) {
			printf("I don't want a message!\n");
			check("sync", connector_sync(c, 1000));
		} else {
			printf("I want a message!\n");
			check("get ", connector_get(c, 0));
			check("sync", connector_sync(c, 1000));
			check("read msg", connector_gotten(c, 0, &msg, &msg_len));
			printf("Got message: `%.*s`\n", msg_len, msg);
		}
	}
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	free(pdl);
	return 0;
}