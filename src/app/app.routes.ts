import { Routes } from '@angular/router';

import { Docs_T01_IntoComponent } from './docs/t01-starter/into-t01-getting-starter/into-title-01.component';
import { Docs_T02_IntoComponent } from './docs/t02-concepts/into-t02-main-concepts/into-title-02.component';
import { Docs_T02_Pag01_Component } from './docs/t02-concepts/t02-page-01/page-01.component';
import { Docs_T02_Pag02_Component } from './docs/t02-concepts/t02-page-02/page-02.component';
import { Docs_T02_Pag03_Component } from './docs/t02-concepts/t02-page-03/page-03.component';
import { Docs_T03_IntoComponent } from './docs/t03-getting-started/into-t03-main-getting/into-title-03.component';

import HomeComponent from './pages/home/home.component';
import { NotFoundComponentComponent } from './pages/error/404/not-found.component';
import { DocsPageComponent } from './docs/page/page.component';

export const routes: Routes = [
  { path: '', component: HomeComponent, // HomeComponent
    children: [
      { path: 'introduction', component: Docs_T01_IntoComponent }, // docs-t01-into-getting-starter
      { path: 'concepts', component: Docs_T02_IntoComponent, // docs-t02-into-main-concepts
        children: [
          { path: 'structure&infrastructure', component: Docs_T02_Pag01_Component }, // doc-t02-mod-01-levels-structure
          { path: 'support-nodes', component: Docs_T02_Pag02_Component }, // doc-t02-mod-02-support-folders
          { path: 'files-nomenclature', component: Docs_T02_Pag03_Component }, // doc-t02-mod-03-nomenclature
          { path: '', redirectTo: 'structure&infrastructure', pathMatch: 'prefix' },
        ] 
      },
      { path: 'getting-started', component: Docs_T03_IntoComponent },
      { path: 'test/:lang/:title', component: DocsPageComponent },
      { path: 'test/:lang/:title/:page', component: DocsPageComponent },
      { path: '', redirectTo: 'introduction', pathMatch: 'full' },
    ]
  },
  { path: 'not-found', component: NotFoundComponentComponent },
  { path: '**', redirectTo: 'not-found', pathMatch: 'full' },
];