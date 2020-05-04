#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../reowolf.h"
#include "../utility.c"

int main() { // AMY
	char * pdl = buffer_pdl("eg_protocols.pdl");
	
	Connector* c = connector_new();
	printf("configuring...\n");

	check("config ", connector_configure(c, pdl, "sync_two"));
	check("bind 0 ", connector_bind_native(c, 0));
	check("bind 1 ", connector_bind_native(c, 1));
	check("bind 2 ", connector_bind_passive(c, 2, "127.0.0.1:7000"));
	check("bind 3 ", connector_bind_passive(c, 3, "127.0.0.1:7001"));
	printf("connecting...\n");
	check("connect", connector_connect(c, 5000));

	int round, code;
	for(round = 0; true; round++) {
		printf("\nround %d\n", round);
	
		check("next ", connector_next_batch(c));
		check("put  ", connector_put(c, 0, "one", 3));
		check("put  ", connector_put(c, 1, "two", 3));
		code = connector_sync(c, 3000);
		if(code == 0)       printf("sent neither message!");
		else if(code == 1 ) printf("sent both messages!");
		else if(code == -1) printf("No interaction! Recovered state.\n");
		else {
			printf("Unrecoverable error!\n");
			connector_dump_log(c);
			break;
		}
	}
	
	printf("cleaning up...\n");
	connector_destroy(c);
	free(pdl);
	return 0;
}