import type { Request, Response } from "express";
export interface User { id: number; name?: string; readonly tags: string[] }

type Result<T, E = Error> =
    | { ok: true; value: T }
    | { ok: false; error: E };

enum Color { Red = 1, Green, Blue }

@Injectable()
export class UserService<T extends User = User> implements Service {
    private cache: Map<number, T> = new Map();
    private ready!: boolean;
    constructor(private readonly http: HttpClient, @Inject(TOKEN) private token?: string) {}

    async find(id: number): Promise<T | undefined> {
        const user = this.cache.get(id) as T | undefined;
        return user ?? await this.http.get<T>(`/users/${id}`);
    }

    get size(): number { return this.cache.size }
    static create<U extends User>(items: U[]): UserService<U> { return new UserService<U>(null!, undefined); }
}

function assertIsString(val: any): asserts val is string {}
const handler = (req: Request, res: Response): void => { res.json({ ok: true }); };
let x: Array<Record<string, number>> = [];
type Keys = keyof typeof obj;
type Mapped = { [K in Keys]?: string };
type Cond<T> = T extends string ? "str" : T extends number ? "num" : never;
let tuple: [string, number?] = ["a"];
declare module "foo" { export const bar: number; }
