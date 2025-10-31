import { requestClient } from '#/api/request';

export namespace AnalyticsApi {

    export interface AnalyticsMonitor {
        time: string;
        cpu: number;
        cpu_current_process: number;
        cpu_load_one: number;
        cpu_load_five: number;
        cpu_load_fifteen: number;
        mem: number;
        swap: number;
        disk: number;
        io_read: number;
        io_written: number;
        net_send: number;
        net_received: number;
        rt: number;
        error: number;
    }

    export interface AnalyticsTraffic {
        data_latest_30: Array<AnalyticsTrafficItem>;
        data_1day: Array<AnalyticsTrafficItem>;
        data_1month: Array<AnalyticsTrafficItem>;
    }

    export interface StringMap {
        [key: string]: number;
    }

    export interface ExtInfo {
        http_country: StringMap;
        http_code: StringMap;
        http_source: StringMap;
    }

    export interface AnalyticsTrafficItem {
        time: string;
        tls: number;
        pv: number;
    }

}

export async function getAnalyticsMonitorApi(name: string): Promise<Array<AnalyticsApi.AnalyticsMonitor>> {
    var arrays = Array();
    const items = await requestClient.get(`/analytics/monitor/${name}`);
    for (const a of items) {
        const r: AnalyticsApi.AnalyticsMonitor = {
            time: a.time,
            cpu: a.item.cpu,
            cpu_current_process: a.item.cpu_current_process,
            cpu_load_one: a.item.cpu_load_one,
            cpu_load_five: a.item.cpu_load_five,
            cpu_load_fifteen: a.item.cpu_load_fifteen,
            mem: a.item.mem,
            swap: a.item.swap,
            disk: a.item.disk,
            io_read: a.item.io_read,
            io_written: a.item.io_written,
            net_send: a.item.net_send,
            net_received: a.item.net_received,
            rt: a.item.rt,
            error: a.item.error,
        };
        arrays.push(r)
    }
    return arrays
}

export async function getAnalyticsMonitorServerApi(name: string, ip: string): Promise<Array<AnalyticsApi.AnalyticsMonitor>> {
    var arrays = Array();
    const items = await requestClient.get(`/analytics/monitor/${name}/${ip}`);
    for (const a of items) {
        const r: AnalyticsApi.AnalyticsMonitor = {
            time: a.time,
            cpu: a.item.cpu,
            cpu_current_process: a.item.cpu_current_process,
            cpu_load_one: a.item.cpu_load_one,
            cpu_load_five: a.item.cpu_load_five,
            cpu_load_fifteen: a.item.cpu_load_fifteen,
            mem: a.item.mem,
            swap: a.item.swap,
            disk: a.item.disk,
            io_read: a.item.io_read,
            io_written: a.item.io_written,
            net_send: a.item.net_send,
            net_received: a.item.net_received,
            rt: a.item.rt,
            error: a.item.error,
        };
        arrays.push(r)
    }
    return arrays
}

export async function getAnalyticsTraffic(name: string): Promise<AnalyticsApi.AnalyticsTraffic> {
    return await requestClient.get(`/analytics/traffic/${name}`);
}

export async function getAnalyticsTrafficExt(name: string): Promise<AnalyticsApi.ExtInfo> {
    return await requestClient.get(`/analytics/traffic/${name}/ext`);
}