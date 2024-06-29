import { BrowserModule, Title } from '@angular/platform-browser';
import { RouterModule, Routes } from '@angular/router';
import HomeComponent from './pages/home/home.component';
import { Docs_T01_IntoComponent } from './docs/t01-starter/into-getting-starter/into-getting-starter.component';
import { Docs_T02_IntoComponent } from './docs/t02-concepts/into-main-concepts/into-main-concepts.component';
import { NgModule } from '@angular/core';
import { HashLocationStrategy, LocationStrategy } from '@angular/common';

export const routes: Routes = [
  {  path: 'home', component: HomeComponent,  
    children: [
      { path: '', redirectTo: 'docs/introduction', pathMatch: 'full' },
      {  path: 'docs/introduction', component: Docs_T01_IntoComponent  },
      {  path: 'docs/concepts', component: Docs_T02_IntoComponent  },
    ]
  },
  { path: '', redirectTo: '/home',  pathMatch: 'full'  }
];

@NgModule({
  imports:[
    BrowserModule,
    RouterModule.forRoot(routes)
  ],
  providers: [
    {
      provide: LocationStrategy,
      useClass: HashLocationStrategy
    }
  ],
})

export class AppRoutingModule { }