#include <stdio.h>
#include "reowolf.h"

int main() {
	Connector* c = connector_new();

	if (connector_configure(c, "primitive main(){}")) {
		printf("CONFIG FAILED\n");
	}
	if (port_bind_native(c, 0)) {
		printf("BIND0 FAILED\n");
	}
	if (port_bind_passive(c, 1, "0.0.0.0:8888")) {
		printf("BIND1 FAILED\n");
	}
	if (port_bind_passive(c, 2, "0.0.0.0:8888")) {
		printf("BIND1 FAILED\n");
	}
	printf("OK\n");
	connector_destroy(c);
	return 0;
}