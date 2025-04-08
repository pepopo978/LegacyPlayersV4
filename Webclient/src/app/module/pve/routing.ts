import {NgModule} from "@angular/core";
import {RouterModule, Routes} from "@angular/router";
import {PveComponent} from "./component/pve/pve";

const routes: Routes = [
    {
        path: "", component: PveComponent, children: [
        {path: "", loadChildren: () => import("./module/search/module").then(m => m.SearchModule)},
        {path: "ranking", loadChildren: () => import("./module/ranking/module").then(m => m.RankingModule)},
        ]
    }
];

@NgModule({
    imports: [RouterModule.forChild(routes)],
    exports: [RouterModule]
})
export class PveRouting {
}
