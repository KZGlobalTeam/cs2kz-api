# How to integrate with this API

This document is meant for people working on [CS2KZ](https://github.com/KZGlobalTeam/cs2kz-metamod).
It is meant to give a rough outline for client requirements such as authentication.

> [!IMPORTANT]
> Some design questions here are still open, so this will likely change.

## Request structure

As this is a [REST API](https://en.wikipedia.org/wiki/REST), you will make HTTP requests.

Some requests require authentication, such as creating or updating players.

Therefore every KZ server will have an **API Key**, which is "permanent", unless it's unused for
a certain period of time. If an **API Key** "expires", the server counts as "unverified" and the
server owner will be notified to reactivate their server.

However, individual requests requires the server to provide a temporary **API Token**.
This token is a [JWT](https://jwt.io/introduction) that is generated using the server's **API Key**.

Each token is valid for 30 minutes, so servers are encouraged to regenerate their tokens every ~25
minutes. This is done by making a `POST` request to `/api/auth/refresh` with the **API Key**. On
success, the generated token will then be sent to the server via UDP, and a `200 OK` is returned via
HTTP. This token will then be included in **every** request (either because the request requires
authentication or just to get better rate limits) as a header. This header is a standard
`Authorization Bearer` header.

Furthermore, every request has to include a `plugin-version` header which contains an integer value
describing the particular version of CS2KZ that the requesting server is running on. This is to
ensure that servers stay up to date.

Anything that is request-specific is documented via [OpenAPI](https://www.openapis.org); the
repository root holds [a JSON file](./api-spec.json) that fully describes the API.

The API also hosts a [SwaggerUI](https://swagger.io/tools/swagger-ui) web page under
`/api/docs/swagger-ui` that you can use to interactively make requests in a web browser.
