import { Injectable } from '@angular/core';
import type { Observable } from 'rxjs';

export interface Todo {
    id: number;
    title: string;
    done: boolean;
    tags?: readonly string[];
}

export type Filter = 'all' | 'open' | 'done';

/** Keeps the list of todos in memory. */
@Injectable({ providedIn: 'root' })
export class TodoStore {
    private items: Todo[] = [];
    private nextId = 1;

    add(title: string, tags: string[] = []): Todo {
        const todo: Todo = { id: this.nextId++, title: title.trim(), done: false, tags };
        this.items = [...this.items, todo];
        return todo;
    }

    toggle(id: number): void {
        this.items = this.items.map((item) => item.id === id ? { ...item, done: !item.done } : item);
    }

    filter(kind: Filter): Todo[] {
        switch (kind) {
            case 'open':
                return this.items.filter((item) => !item.done);
            case 'done':
                return this.items.filter((item) => item.done);
            default:
                return [...this.items];
        }
    }

    get remaining(): number {
        return this.items.reduce((count, item) => (item.done ? count : count + 1), 0);
    }
}

export function groupBy<T, K extends string | number>(values: readonly T[], key: (value: T) => K): Record<K, T[]> {
    const result = {} as Record<K, T[]>;
    for (const value of values) {
        (result[key(value)] ??= []).push(value);
    }
    return result;
}
