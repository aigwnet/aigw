
import { z } from '#/adapter/form';
import * as ipaddr from "ipaddr.js";

function isValidIpOrCidr(str: string): boolean {
    try {
        if (str.includes('/')) {
            const [addr, prefix] = str.split('/');
            const ip = ipaddr.parse(addr!);
            const max = ip.kind() === 'ipv4' ? 32 : 128;
            const pre = parseInt(prefix!, 10);
            return !isNaN(pre) && pre >= 0 && pre <= max;
        } else {
            ipaddr.parse(str);
            return true;
        }
    } catch {
        return false;
    }
}

export const ipListRule = z.string().refine(
    (value) =>
        value
            .split(/\r?\n/)
            .map(l => l.trim())
            .filter(l => l !== '')
            .every(isValidIpOrCidr),
    {
        message: 'Each line must be a valid IPv4 or IPv6 address (with optional CIDR)'
    }
);