declare module "node:fs/promises" {
  export function mkdir(
    path: string,
    options: { recursive: true },
  ): Promise<string | undefined>;
  export function writeFile(path: string, data: string, encoding: "utf8"): Promise<void>;
}

declare module "node:path" {
  export function dirname(path: string): string;
  export function resolve(...paths: string[]): string;
}

declare const process: {
  argv: string[];
  stdout: { write(value: string): void };
};
