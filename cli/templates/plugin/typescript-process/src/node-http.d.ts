declare module "node:http" {
  export type IncomingMessage = {
    headers: Record<string, string | string[] | undefined>;
    method?: string;
    url?: string;
    on(event: "data", listener: (chunk: Uint8Array) => void): IncomingMessage;
    on(event: "end", listener: () => void): IncomingMessage;
    on(event: "error", listener: (error: Error) => void): IncomingMessage;
    destroy(): void;
  };

  export type ServerResponse = {
    setHeader(name: string, value: string | number): void;
    writeHead(status: number): void;
    end(body?: string): void;
  };

  export type Server = {
    listen(port: number, host: string, callback: () => void): void;
    close(callback: () => void): void;
  };

  export function createServer(
    listener: (request: IncomingMessage, response: ServerResponse) => void,
  ): Server;
}

declare const process: {
  env: Record<string, string | undefined>;
  on(event: "SIGTERM" | "SIGINT", listener: () => void): void;
};
