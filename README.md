# ua-detect

A gRPC server and client for validating user agents.

## Protobuf

Our protobuf definition is available in [crates/validate/proto/validate.proto](crates/validate/proto/validate.proto) and you can also consume `ua-detect-validate` for generated Rust bindings.

## Server

We also provide a server component that can be installed using `cargo install --path crates/server` or `nix profile install github:phated/ua-detect`  (both client and server will be installed by Nix).

This will make the `ua-detect-server` binary available in your path. Running it will start the gRPC server in default mode, but additional flags may be specified to configure it.

```sh
$ ua-detect-server --help
Start a gRPC server to validate user agents

Usage: ua-detect-server [OPTIONS]

Options:
      --ip <ip>      The IP address the server will bind to - e.g. 127.0.0.1 [default: [::1]]
      --port <port>  The port the server will listen on [default: 9001]
  -h, --help         Print help
  -V, --version      Print version
```

## Client

The client component can be used to test requests against the server and can be installed using `cargo install --path crates/client` or `nix profile install github:phated/ua-detect` (both client and server will be installed by Nix).

The client allows you to specify a full User Agent to validate, but also allows for the `chrome`, `firefox`, `safari`, and `opera` shorthands. If no User Agent is specified, a random one will be generated.

```sh
$ ua-detect-client --help
Communicate with the ua-detect gRPC server

Usage: ua-detect-client [OPTIONS] [user-agent]

Arguments:
  [user-agent]  The full user-agent to validate or a shorthand helper [chrome, firefox, safari, opera]

Options:
      --url <url>  The url to the running ua-detect service [default: http://[::1]:9001]
  -h, --help       Print help
  -V, --version    Print version
```

## License

MPL-2.0
