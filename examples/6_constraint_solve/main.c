#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "../../reowolf.h"
#include "../utility.c"

int main(int arg_c, char * argv[]) {
	int index;
	if (arg_c != 2) {
		printf("Expected one arg: which peer I am in 0..4");
		return 1;
	}
	index = atoi(argv[1]);
	printf("I am peer %d\n", index);
	
	const char* addrs[] = {
		"127.0.0.1:7000",
		"127.0.0.1:7001",
		"127.0.0.1:7002",
		"127.0.0.1:7003",
		"127.0.0.1:7004",
		"127.0.0.1:7005",
	};

	char * pdl = buffer_pdl("eg_protocols.pdl");
	
	Connector* c = connector_new();
	printf("configuring...\n");

	check("config ", connector_configure(c, pdl, "xor_three"));
	int i, j;
	int addr_index = 0;
	int port = 0;
	for (i = 0; i < 4; i++) {
		for (j = i+1; j < 4; j++) {
			if (i==index) {
				printf("ports %d and %d are for a passive channel to peer %d over addr %s\n", port, port+1, j, addrs[addr_index]);
				check("bind an ", connector_bind_native(c, port));
				port++;
				check("bind a  ", connector_bind_active(c, port, addrs[addr_index]));
				port++;
			} else if (j==index) {
				printf("ports %d and %d are for an active channel to peer %d over addr %s\n", port, port+1, i, addrs[addr_index]);
				check("bind p  ", connector_bind_passive(c, port, addrs[addr_index]));
				port++;
				check("bind pn ", connector_bind_native(c, port));
				port++;
			}
			addr_index++;
		}
	}
	check("connect", connector_connect(c, 5000));
	
	for (i = 0; i < 4; i++) {
		if (i == index) continue;
		// another batch
		for (j = 0; j < 4; j++) {
			
		}
	}
	connector_sync();
	
	printf("destroying...\n");
	connector_destroy(c);
	printf("exiting...\n");
	free(pdl);
	return 0;
}