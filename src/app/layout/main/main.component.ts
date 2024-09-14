import { Component } from '@angular/core';
import { FooterComponent } from '../footer/footer.component';
import { Docs_T01_IntoComponent } from '../../docs/t01-starter/into-getting-starter/into-title-01.component';
import { Docs_T02_IntoComponent } from '../../docs/t02-concepts/into-t02-main-concepts/into-title-02.component';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-main',
  standalone: true,
  imports: [
    RouterOutlet,
  ],
  templateUrl: './main.component.html',
  styleUrl: './main.component.css'
})
export class MainComponent {

}
