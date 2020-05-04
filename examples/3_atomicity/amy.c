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

		check("put  ", connector_put(c, 0, "one", 3));
		check("put  ", connector_put(c, 1, "two", 3));
		check("sync ", connector_sync(c, 10000));
	}
	
	printf("cleaning up...\n");
	connector_destroy(c);
	free(pdl);
	return 0;
}