import { getLang } from "./store";



const Slovar: Record<string, string[]> = {
    'nav-feed': [ 'for you', 'для вас' ],
    'nav-dm': ['DM', 'личка'],
    'nav-chats': ['chats', 'чаты'],
    'nav-sets': ['settings', 'настройки'],

}


export function rep(str: string): string {
    switch (getLang()){
        case 'ru':
            return Slovar[str][1];
        default:
            return Slovar[str][0];
    }
}

