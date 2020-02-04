/* CBindgen generated */

#ifndef REOWOLF_HEADER_DEFINED
#define REOWOLF_HEADER_DEFINED

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Connector Connector;

typedef uint32_t ControllerId;

/**
 * Configures the given Reowolf connector with a protocol description in PDL.
 * Returns:
 */
int connector_configure(Connector *connector, char *pdl);

/**
 * Provides a binding annotation for the port with the given index with "active":
 * (The port will conenct to a "passive" port at the given address during connect())
 * Returns:
 * - 0 SUCCESS: connected successfully
 * - TODO error codes
 */
int connector_connect(Connector *connector, uint64_t timeout_millis);

/**
 * Destroys the given connector, freeing its underlying resources.
 */
void connector_destroy(Connector *connector);

/**
 * Resets the error message buffer.
 * Returns:
 * - 0 if an error was cleared
 * - 1 if there was no error to clear
 */
int connector_error_clear(void);

/**
 * Returns a pointer into the error buffer for reading as a null-terminated string
 * Returns null if there is no error in the buffer.
 */
const char *connector_error_peek(void);

/**
 * Creates and returns Reowolf Connector structure allocated on the heap.
 */
Connector *connector_new(void);

int connector_next_batch(Connector *connector);

int connector_sync(Connector *connector, uint64_t timeout_millis);

/**
 * Creates and returns Reowolf Connector structure allocated on the heap.
 */
Connector *connector_with_controller_id(ControllerId controller_id);

/**
 * Provides a binding annotation for the port with the given index with "active":
 * (The port will conenct to a "passive" port at the given address during connect())
 * Returns:
 * - 0 for success
 * - 1 if the port was already bound and was left unchanged
 */
int port_bind_active(Connector *connector, unsigned int proto_port_index, const char *address);

/**
 * Provides a binding annotation for the port with the given index with "native":
 * (The port is exposed for reading and writing from the application)
 * Returns:
 */
int port_bind_native(Connector *connector, uintptr_t proto_port_index);

/**
 * Provides a binding annotation for the port with the given index with "native":
 * (The port is exposed for reading and writing from the application)
 * Returns:
 */
int port_bind_passive(Connector *connector, unsigned int proto_port_index, const char *address);

int port_close(Connector *connector, unsigned int _proto_port_index);

/**
 * Prepares to synchronously put a message at the given port, writing it to the given buffer.
 * - 0 SUCCESS
 * - 1 this port has the wrong direction
 * - 2 this port is already marked to get
 */
int port_get(Connector *connector, unsigned int proto_port_index);

/**
 * Prepares to synchronously put a message at the given port, reading it from the given buffer.
 */
int port_put(Connector *connector,
             unsigned int proto_port_index,
             unsigned char *buf_ptr,
             unsigned int msg_len);

int read_gotten(Connector *connector,
                unsigned int proto_port_index,
                const unsigned char **buf_ptr_outptr,
                unsigned int *len_outptr);

#endif /* REOWOLF_HEADER_DEFINED */
