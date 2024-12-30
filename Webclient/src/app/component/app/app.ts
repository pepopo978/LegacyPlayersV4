import {AfterViewInit, Component, OnChanges, OnInit} from "@angular/core";
import {NavigationEnd, Router} from "@angular/router";
import {Subscription} from "rxjs";
import {SettingsService} from "../../service/settings";
import {TranslationService} from "../../service/translation";
import {APIService} from "../../service/api";
import {AccountInformation} from "../../module/account/domain_value/account_information";

declare var gtag;

@Component({
    selector: "root",
    templateUrl: "./app.html",
    styleUrls: ["./app.scss"]
})
export class AppComponent implements OnInit, OnChanges {
    private static readonly URL_ACCOUNT_GET: string = '/account/get';


    title = "LegacyPlayers";

    enable_ads: boolean = false;
    is_on_viewer_site: boolean = false;
    enough_bottom_space: boolean = false;
    enough_width_for_side_ads: boolean = false;

    constructor(
        private settingsService: SettingsService,
        private translationService: TranslationService,
        private router: Router,
        private apiService: APIService
    ) {
        (window as any).addEventListener("beforeinstallprompt", (e) => () => this.prompt_for_pwa(e));
        this.router.events.subscribe(event => this.set_ad_width_flags());
    }

    ngOnInit(): void {
        this.retrieve_account_information();
        setInterval(() => this.set_ad_width_flags(), 500);
    }

    ngOnChanges(): void {
    }

    set_ad_width_flags(): void {
        this.is_on_viewer_site = this.router.url.includes("viewer/");
        this.enough_width_for_side_ads = document.getElementsByTagName("body")[0].clientWidth >= 800;
        const ad_element = document.getElementById("bottom_layer");
        this.enough_bottom_space = !!ad_element && ad_element.clientWidth >= 2000;
    }

    private prompt_for_pwa(e: any): void {
        if (this.settingsService.check("PWA_PROMPT"))
            return;
        e.prompt();
        this.settingsService.set("PWA_PROMPT", true);
    }

    get isMobile(): boolean {
        return navigator.userAgent.toLowerCase().includes("mobile") || !this.enough_width_for_side_ads;
    }

    private retrieve_account_information(): void {
        if (this.settingsService.check("API_TOKEN")) {
            this.apiService.get<AccountInformation>(AppComponent.URL_ACCOUNT_GET, (result) => {
                this.settingsService.set("ACCOUNT_INFORMATION", result);
                this.enable_ads = !(((result as AccountInformation).access_rights & 2) == 2);
            });
        }
    }
}
