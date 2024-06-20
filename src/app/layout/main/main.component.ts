import { Component } from '@angular/core';
import { FooterComponent } from '../footer/footer.component';
import { Docs_T01_IntoComponent } from '../../docs/t01-starter/into-getting-starter/into-getting-starter.component';
import { Docs_T02_IntoComponent } from '../../docs/t02-concepts/into-main-concepts/into-main-concepts.component';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-main',
  standalone: true,
  imports: [
    Docs_T01_IntoComponent,
    RouterOutlet,
  ],
  templateUrl: './main.component.html',
  styleUrl: './main.component.css'
})
export class MainComponent {

}
